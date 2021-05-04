def to_ms():
	f = open("gpu")
	lines = f.readlines()
	for j, l in enumerate(lines):
		l = l.split()
		#print(l)
		for i in range(1, len(l)):
			if l[i].isnumeric():
				l[i] = str(int(l[i])/100)
			if l[i] == "us":
				l[i] = ' ms '
			if l[i] == '&':
				l[i] = '& '

		if j % 3 == 0:
			l.append('& ')
			l.append(str(float(l[12])/float(l[6]))[:5])
			l.append(' & ')
			l.append(str(float(l[12+3])/float(l[6+3][:5])))
		else:
			l.append('& ')
			l.append(str(float(l[11])/float(l[5]))[:5])
			l.append(' & ')
			l.append(str(float(l[11+3])/float(l[5+3]))[:5])
		lines[j] = "".join(l)
		print(lines[j])


to_ms()

